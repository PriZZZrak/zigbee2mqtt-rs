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
exports.generateDefinition = generateDefinition;
exports.generateGreenPowerDefinition = generateGreenPowerDefinition;
const zigbee_herdsman_1 = require("zigbee-herdsman");
const consts_1 = require("zigbee-herdsman/dist/zspec/zcl/definition/consts");
const logger_1 = require("./logger");
const m = __importStar(require("./modernExtend"));
const philips = __importStar(require("./philips"));
const utils_1 = require("./utils");
const NS = "zhc:gendef";
// Generator allows to define instances of GeneratedExtend that have typed arguments to extender.
class ExtendGenerator {
    extend;
    args;
    source;
    lib;
    multiEndpointSkip;
    constructor(args) {
        this.extend = args.extend;
        this.args = args.args;
        this.source = args.source;
        this.lib = args.lib;
        this.multiEndpointSkip = args.multiEndpointSkip;
    }
    getExtend() {
        return this.extend(this.args);
    }
    getSource() {
        let jsonArgs = JSON.stringify(this.args);
        if (!this.args || jsonArgs === "{}") {
            jsonArgs = "";
        }
        return `${this.source}(${jsonArgs})`;
    }
}
// If generator will have endpoint argument - generator implementation
// should not provide it if only the first device endpoint is passed in.
// If multiple endpoints provided(maybe including the first device endpoint) -
// they all should be passed as an argument, where possible, to be explicit.
const INPUT_EXTENDERS = [
    [
        ["msTemperatureMeasurement"],
        async (d, eps) => [new ExtendGenerator({ extend: m.temperature, args: maybeEndpointArgs(d, eps), source: "temperature" })],
    ],
    [["msPressureMeasurement"], async (d, eps) => [new ExtendGenerator({ extend: m.pressure, args: maybeEndpointArgs(d, eps), source: "pressure" })]],
    [["msRelativeHumidity"], async (d, eps) => [new ExtendGenerator({ extend: m.humidity, args: maybeEndpointArgs(d, eps), source: "humidity" })]],
    [["msCO2"], async (d, eps) => [new ExtendGenerator({ extend: m.co2, args: maybeEndpointArgs(d, eps), source: "co2" })]],
    [["genPowerCfg"], async (d, eps) => [new ExtendGenerator({ extend: m.battery, source: "battery" })]],
    [["genOnOff", "genLevelCtrl", "lightingColorCtrl"], extenderOnOffLight],
    [["seMetering", "haElectricalMeasurement"], extenderElectricityMeter],
    [["closuresDoorLock"], extenderLock],
    [
        ["msIlluminanceMeasurement"],
        async (d, eps) => [new ExtendGenerator({ extend: m.illuminance, args: maybeEndpointArgs(d, eps), source: "illuminance" })],
    ],
    [["msOccupancySensing"], async (d, eps) => [new ExtendGenerator({ extend: m.occupancy, args: maybeEndpointArgs(d, eps), source: "occupancy" })]],
    [
        ["ssIasZone"],
        async (d, eps) => [
            new ExtendGenerator({
                extend: m.iasZoneAlarm,
                args: {
                    zoneType: "generic",
                    zoneAttributes: ["alarm_1", "alarm_2", "tamper", "battery_low"],
                },
                source: "iasZoneAlarm",
            }),
        ],
    ],
    [["ssIasWd"], async (d, eps) => [new ExtendGenerator({ extend: m.iasWarning, source: "iasWarning" })]],
    [
        ["genDeviceTempCfg"],
        async (d, eps) => [new ExtendGenerator({ extend: m.deviceTemperature, args: maybeEndpointArgs(d, eps), source: "deviceTemperature" })],
    ],
    [["pm25Measurement"], async (d, eps) => [new ExtendGenerator({ extend: m.pm25, args: maybeEndpointArgs(d, eps), source: "pm25" })]],
    [["msFlowMeasurement"], async (d, eps) => [new ExtendGenerator({ extend: m.flow, args: maybeEndpointArgs(d, eps), source: "flow" })]],
    [["msSoilMoisture"], async (d, eps) => [new ExtendGenerator({ extend: m.soilMoisture, args: maybeEndpointArgs(d, eps), source: "soilMoisture" })]],
    [
        ["closuresWindowCovering"],
        async (d, eps) => [
            new ExtendGenerator({
                extend: m.windowCovering,
                args: { controls: ["lift", "tilt"] },
                source: "windowCovering",
                multiEndpointSkip: ["state", "position", "tilt", "cover_mode"],
            }),
        ],
    ],
    [["genBinaryInput"], extenderBinaryInput],
    [["genBinaryOutput"], extenderBinaryOutput],
    [["genAnalogInput"], extenderAnalogInput],
    [["genAnalogOutput"], extenderAnalogOutput],
    [["hvacThermostat"], extenderThermostat],
    [["hvacFanCtrl"], extenderFanControl],
    [["hvacUserInterfaceCfg"], extenderThermostatUi],
    [["genIdentify"], async (d, eps) => [new ExtendGenerator({ extend: m.identify, source: "identify" })]],
    [["lightingBallastCfg"], async (d, eps) => [new ExtendGenerator({ extend: m.lightingBallast, source: "lightingBallast" })]],
];
const OUTPUT_EXTENDERS = [
    [
        ["genOnOff"],
        async (d, eps) => [
            new ExtendGenerator({
                extend: m.commandsOnOff,
                args: maybeEndpointArgs(d, eps),
                source: "commandsOnOff",
                multiEndpointSkip: onlyFirstDeviceEnpoint(d, eps) ? ["on", "off", "toggle"] : undefined,
            }),
        ],
    ],
    [
        ["genLevelCtrl"],
        async (d, eps) => [
            new ExtendGenerator({
                extend: m.commandsLevelCtrl,
                args: maybeEndpointArgs(d, eps),
                source: "commandsLevelCtrl",
                multiEndpointSkip: onlyFirstDeviceEnpoint(d, eps)
                    ? [
                        "brightness_move_to_level",
                        "brightness_move_up",
                        "brightness_move_down",
                        "brightness_step_up",
                        "brightness_step_down",
                        "brightness_stop",
                    ]
                    : undefined,
            }),
        ],
    ],
    [
        ["lightingColorCtrl"],
        async (d, eps) => [
            new ExtendGenerator({
                extend: m.commandsColorCtrl,
                args: maybeEndpointArgs(d, eps),
                source: "commandsColorCtrl",
                multiEndpointSkip: onlyFirstDeviceEnpoint(d, eps)
                    ? [
                        "color_temperature_move_stop",
                        "color_temperature_move_up",
                        "color_temperature_move_down",
                        "color_temperature_step_up",
                        "color_temperature_step_down",
                        "enhanced_move_to_hue_and_saturation",
                        "move_to_hue_and_saturation",
                        "color_hue_step_up",
                        "color_hue_step_down",
                        "color_saturation_step_up",
                        "color_saturation_step_down",
                        "color_loop_set",
                        "color_temperature_move",
                        "color_move",
                        "hue_move",
                        "hue_stop",
                        "move_to_saturation",
                        "move_to_hue",
                        "stop_move_step",
                    ]
                    : undefined,
            }),
        ],
    ],
    [
        ["closuresWindowCovering"],
        async (d, eps) => [
            new ExtendGenerator({
                extend: m.commandsWindowCovering,
                args: maybeEndpointArgs(d, eps),
                source: "commandsWindowCovering",
                multiEndpointSkip: onlyFirstDeviceEnpoint(d, eps) ? ["open", "close", "stop"] : undefined,
            }),
        ],
    ],
];
function generateSource(definition, generatedExtend) {
    const imports = [...new Set(generatedExtend.map((e) => e.lib ?? "modernExtend"))];
    const importsStr = imports.map((e) => `import * as ${e === "modernExtend" ? "m" : e} from 'zigbee-herdsman-converters/lib/${e}';`).join("\n");
    const meta = definition.meta ? `\n    meta: ${JSON.stringify(definition.meta)},` : "";
    return `${importsStr}

export default {
    zigbeeModel: ['${definition.zigbeeModel}'],
    model: '${definition.model}',
    vendor: '${definition.vendor}',
    description: 'Automatically generated definition',
    extend: [${generatedExtend.map((e) => `${e.lib ?? "m"}.${e.getSource()}`).join(", ")}],${meta}
};
`;
}
function generateGreenPowerSource(definition, ieeeAddr) {
    return `import {genericGreenPower} from 'zigbee-herdsman-converters/lib/modernExtend';

export default {
    fingerprint: [{modelID: '${definition.model}', ieeeAddr: new RegExp('^${ieeeAddr}$')}],
    model: '${definition.model}',
    vendor: '${definition.vendor}',
    description: 'Automatically generated definition for Green Power',
    extend: [genericGreenPower()],
};`;
}
async function generateDefinition(device) {
    if (device.type === "GreenPower") {
        return generateGreenPowerDefinition(device);
    }
    // Map cluster to all endpoints that have this cluster.
    const mapClusters = (endpoint, clusters, clusterMap) => {
        for (const cluster of clusters) {
            if (!clusterMap.has(cluster.name)) {
                clusterMap.set(cluster.name, []);
            }
            const endpointsWithCluster = clusterMap.get(cluster.name);
            endpointsWithCluster.push(endpoint);
        }
    };
    const knownInputClusters = INPUT_EXTENDERS.flatMap((ext) => ext[0]);
    const knownOutputClusters = OUTPUT_EXTENDERS.flatMap((ext) => ext[0]);
    const inputClusterMap = new Map();
    const outputClusterMap = new Map();
    for (const endpoint of device.endpoints) {
        // Filter clusters to leave only the ones that we can generate extenders for.
        const inputClusters = endpoint.getInputClusters().filter((c) => knownInputClusters.find((known) => known === c.name));
        const outputClusters = endpoint.getOutputClusters().filter((c) => knownOutputClusters.find((known) => known === c.name));
        mapClusters(endpoint, inputClusters, inputClusterMap);
        mapClusters(endpoint, outputClusters, outputClusterMap);
    }
    // Generate extenders
    const usedExtenders = [];
    const generatedExtend = [];
    const addGenerators = async (clusterName, endpoints, extenders) => {
        const extender = extenders.find((e) => e[0].includes(clusterName));
        if (!extender || usedExtenders.includes(extender)) {
            return;
        }
        usedExtenders.push(extender);
        generatedExtend.push(...(await extender[1](device, endpoints)));
    };
    for (const [cluster, endpoints] of inputClusterMap) {
        await addGenerators(cluster, endpoints, INPUT_EXTENDERS);
    }
    for (const [cluster, endpoints] of outputClusterMap) {
        await addGenerators(cluster, endpoints, OUTPUT_EXTENDERS);
    }
    const extenders = generatedExtend.map((e) => e.getExtend());
    // Generated definition below will provide this.
    for (const extender of extenders) {
        extender.endpoint = undefined;
    }
    // Currently multiEndpoint is enabled if device has more then 1 endpoint.
    // It is possible to better check if device should be considered multiEndpoint
    // based, for example, on generator arguments(i.e. presence of "endpointNames"),
    // but this will be enough for now.
    const endpointsWithoutGreenPower = device.endpoints.filter((e) => e.ID !== 242);
    const multiEndpoint = endpointsWithoutGreenPower.length > 1;
    if (multiEndpoint) {
        const endpoints = {};
        for (const endpoint of endpointsWithoutGreenPower) {
            endpoints[endpoint.ID.toString()] = endpoint.ID;
        }
        // Collect multiEndpointSkip from extends that declare it.
        const multiEndpointSkipArr = generatedExtend.flatMap((e) => e.multiEndpointSkip ?? []);
        const multiEndpointSkip = multiEndpointSkipArr.length ? multiEndpointSkipArr : undefined;
        // Add to beginning for better visibility.
        generatedExtend.unshift(new ExtendGenerator({ extend: m.deviceEndpoints, args: { endpoints, multiEndpointSkip }, source: "deviceEndpoints" }));
        extenders.unshift(generatedExtend[0].getExtend());
    }
    const definition = {
        zigbeeModel: [device.modelID],
        model: device.modelID ?? "",
        vendor: device.manufacturerName ?? "",
        description: "Automatically generated definition",
        extend: extenders,
        generated: true,
    };
    const externalDefinitionSource = generateSource(definition, generatedExtend);
    return { externalDefinitionSource, definition };
}
function generateGreenPowerDefinition(device) {
    const definition = {
        fingerprint: [{ modelID: device.modelID, ieeeAddr: new RegExp(`^${device.ieeeAddr}$`) }],
        model: device.modelID ?? "",
        vendor: device.manufacturerName ?? "",
        description: "Automatically generated definition for Green Power",
        extend: [m.genericGreenPower()],
        generated: true,
    };
    const externalDefinitionSource = generateGreenPowerSource(definition, device.ieeeAddr);
    return { externalDefinitionSource, definition };
}
function stringifyEps(endpoints) {
    return endpoints.map((e) => e.ID.toString());
}
// This function checks if provided array of endpoints contain
// only first device endpoint, which is passed in as `firstEndpoint`.
function onlyFirstDeviceEnpoint(device, endpoints) {
    return endpoints.length === 1 && endpoints[0].ID === device.endpoints[0].ID;
}
// maybeEndpoints returns either `toExtend` if only first device endpoint is provided
// as `endpoints`, or `endpointNames` with `toExtend`.
// This allows to drop unnecessary `endpointNames` argument if it is not needed.
function maybeEndpointArgs(device, endpoints, toExtend) {
    if (onlyFirstDeviceEnpoint(device, endpoints)) {
        return toExtend;
    }
    return { endpointNames: stringifyEps(endpoints), ...toExtend };
}
async function extenderLock(device, endpoints) {
    // TODO: Support multiple endpoints
    if (endpoints.length > 1) {
        logger_1.logger.warning("extenderLock can accept only one endpoint", NS);
    }
    const endpoint = endpoints[0];
    const pinCodeCount = await (0, utils_1.getClusterAttributeValue)(endpoint, "closuresDoorLock", "numOfPinUsersSupported", 50);
    return [new ExtendGenerator({ extend: m.lock, args: { pinCodeCount }, source: "lock" })];
}
async function extenderOnOffLight(device, endpoints) {
    const generated = [];
    const lightEndpoints = endpoints.filter((e) => e.supportsInputCluster("lightingColorCtrl") || e.supportsInputCluster("genLevelCtrl"));
    const onOffEndpoints = endpoints.filter((e) => lightEndpoints.findIndex((ep) => e.ID === ep.ID) === -1);
    if (onOffEndpoints.length !== 0) {
        let endpointNames;
        if (!onlyFirstDeviceEnpoint(device, onOffEndpoints)) {
            endpointNames = onOffEndpoints.map((e) => e.ID.toString());
        }
        generated.push(new ExtendGenerator({
            extend: m.onOff,
            args: { endpointNames },
            source: "onOff",
            multiEndpointSkip: onlyFirstDeviceEnpoint(device, onOffEndpoints) ? ["state", "on", "off", "power_on_behavior"] : undefined,
        }));
    }
    for (const endpoint of lightEndpoints) {
        let endpointNames;
        let multiEndpointSkip;
        if (!onlyFirstDeviceEnpoint(device, lightEndpoints)) {
            endpointNames = lightEndpoints.map((e) => e.ID.toString());
        }
        else {
            multiEndpointSkip = ["state", "on", "off", "brightness", "level_config", "power_on_behavior"];
        }
        // In case read fails, support all features with 31
        let colorCapabilities = 0;
        if (endpoint.supportsInputCluster("lightingColorCtrl")) {
            colorCapabilities = await (0, utils_1.getClusterAttributeValue)(endpoint, "lightingColorCtrl", "colorCapabilities", 31);
        }
        const supportsHueSaturation = (colorCapabilities & (1 << 0)) > 0;
        const supportsEnhancedHueSaturation = (colorCapabilities & (1 << 1)) > 0;
        const supportsColorXY = (colorCapabilities & (1 << 3)) > 0;
        const supportsColorTemperature = (colorCapabilities & (1 << 4)) > 0;
        const args = {};
        args.endpointNames = endpointNames;
        if (supportsColorTemperature) {
            const minColorTemp = await (0, utils_1.getClusterAttributeValue)(endpoint, "lightingColorCtrl", "colorTempPhysicalMin", 150);
            const maxColorTemp = await (0, utils_1.getClusterAttributeValue)(endpoint, "lightingColorCtrl", "colorTempPhysicalMax", 500);
            args.colorTemp = { range: [minColorTemp, maxColorTemp] };
            if (multiEndpointSkip) {
                multiEndpointSkip.push("color_temp", "color_temp_startup");
            }
        }
        if (supportsColorXY) {
            args.color = true;
            if (supportsHueSaturation || supportsEnhancedHueSaturation) {
                args.color = {};
                if (supportsHueSaturation)
                    args.color.modes = ["xy", "hs"];
                if (supportsEnhancedHueSaturation)
                    args.color.enhancedHue = true;
            }
            if (multiEndpointSkip) {
                multiEndpointSkip.push("color", "color_mode");
            }
        }
        if ((supportsColorXY || supportsColorTemperature) && multiEndpointSkip) {
            multiEndpointSkip.push("color_mode", "", "color_options");
        }
        if (endpoint.getDevice().manufacturerID === zigbee_herdsman_1.Zcl.ManufacturerCode.SIGNIFY_NETHERLANDS_B_V) {
            generated.push(new ExtendGenerator({ extend: philips.m.light, args, source: "m.light", lib: "philips" }));
        }
        else {
            generated.push(new ExtendGenerator({ extend: m.light, args, source: "light", multiEndpointSkip }));
        }
    }
    return generated;
}
async function extenderElectricityMeter(device, endpoints) {
    // TODO: Support multiple endpoints
    if (endpoints.length > 1) {
        logger_1.logger.warning("extenderElectricityMeter can accept only one endpoint", NS);
    }
    const endpoint = endpoints[0];
    const metering = endpoint.supportsInputCluster("seMetering");
    const electricalMeasurements = endpoint.supportsInputCluster("haElectricalMeasurement");
    const args = {};
    if (!metering || !electricalMeasurements) {
        args.cluster = metering ? "metering" : "electrical";
    }
    if (args.cluster === "electrical") {
        // If this value will be 0 then the value of 'args.electricalMeasurementType' will not be changed.
        const measurementType = await (0, utils_1.getClusterAttributeValue)(endpoint, "haElectricalMeasurement", "measurementType", 0);
        // MeasurementType will have bit on index 6 set for DC measurement.
        const isDCMeasureType = ((measurementType >> 6) & 1) === 1;
        args.electricalMeasurementType = "ac";
        if (isDCMeasureType) {
            args.electricalMeasurementType = "dc";
        }
    }
    if (endpoint.ID !== 1) {
        args.endpointNames = stringifyEps([endpoint]);
    }
    return [new ExtendGenerator({ extend: m.electricityMeter, args, source: "electricityMeter" })];
}
const THERMOSTAT_CTRL_SEQ_OF_OPER_META = "ctrlSeqeOfOper";
async function extenderThermostat(device, endpoints) {
    const generated = [];
    for (const endpoint of endpoints) {
        const isFirstEndpoint = onlyFirstDeviceEnpoint(device, endpoints);
        // Read ctrlSeqeOfOper to determine thermostat type
        // ZCL values: 0=cooling_only, 1=cooling_with_reheat, 2=heating_only, 3=heating_with_reheat,
        // 4=heating_cooling_4_pipes, 5=heating_cooling_4_pipes_with_reheat
        // The value is persisted in device meta, so on subsequent generations (e.g. after a
        // restart) it is not read from the device again.
        let ctrlSeqeOfOper;
        const storedCtrlSeqeOfOper = device.meta?.[THERMOSTAT_CTRL_SEQ_OF_OPER_META]?.[endpoint.ID];
        if (typeof storedCtrlSeqeOfOper === "number") {
            ctrlSeqeOfOper = storedCtrlSeqeOfOper;
        }
        else {
            const readCtrlSeqeOfOper = await (0, utils_1.getClusterAttributeValue)(endpoint, "hvacThermostat", "ctrlSeqeOfOper", undefined);
            if (typeof readCtrlSeqeOfOper === "number") {
                ctrlSeqeOfOper = readCtrlSeqeOfOper;
                if (!device.meta)
                    device.meta = {};
                if (typeof device.meta[THERMOSTAT_CTRL_SEQ_OF_OPER_META] !== "object" || device.meta[THERMOSTAT_CTRL_SEQ_OF_OPER_META] === null) {
                    device.meta[THERMOSTAT_CTRL_SEQ_OF_OPER_META] = {};
                }
                device.meta[THERMOSTAT_CTRL_SEQ_OF_OPER_META][endpoint.ID] = readCtrlSeqeOfOper;
            }
            else {
                // Read failed - use the default without persisting, so a transient failure
                // does not bake the fallback value into the meta.
                ctrlSeqeOfOper = 2;
            }
        }
        const coolingOnly = ctrlSeqeOfOper === 0 || ctrlSeqeOfOper === 1;
        const heatingAndCooling = ctrlSeqeOfOper === 4 || ctrlSeqeOfOper === 5;
        let setpoints;
        if (coolingOnly) {
            setpoints = { occupiedCoolingSetpoint: { min: 7, max: 30, step: 0.5 } };
        }
        else if (heatingAndCooling) {
            setpoints = {
                occupiedHeatingSetpoint: { min: 7, max: 30, step: 0.5 },
                occupiedCoolingSetpoint: { min: 7, max: 30, step: 0.5 },
            };
        }
        else {
            // heating_only (2, 3 or unknown - default)
            setpoints = { occupiedHeatingSetpoint: { min: 7, max: 30, step: 0.5 } };
        }
        let systemModeValues;
        if (coolingOnly) {
            systemModeValues = ["off", "cool"];
        }
        else if (heatingAndCooling) {
            systemModeValues = ["off", "heat", "cool", "auto"];
        }
        else {
            systemModeValues = ["off", "heat"];
        }
        const thermostatArgs = {
            localTemperature: {},
            runningState: { values: heatingAndCooling ? ["idle", "heat", "cool"] : coolingOnly ? ["idle", "cool"] : ["idle", "heat"] },
            setpoints: { values: setpoints },
            systemMode: { values: systemModeValues },
        };
        if (!isFirstEndpoint) {
            thermostatArgs.endpoint = endpoint.ID.toString();
        }
        // Check if hvacFanCtrl is on the same endpoint - add fanMode if so
        if (endpoint.supportsInputCluster("hvacFanCtrl")) {
            thermostatArgs.fanMode = ["off", "low", "medium", "high", "auto"];
        }
        generated.push(new ExtendGenerator({ extend: m.thermostat, args: thermostatArgs, source: "thermostat" }));
    }
    // When the thermostat is on only one endpoint but the device has multiple,
    // declare its properties to be skipped from endpoint suffixing.
    // This prevents fz.thermostat from producing suffixed keys that clash with
    // the unsuffixed keys returned by tz.thermostat_* converters.
    const endpointsWithoutGreenPower = device.endpoints.filter((e) => e.ID !== 242);
    if (endpoints.length === 1 && endpointsWithoutGreenPower.length > 1) {
        generated[0].multiEndpointSkip = [
            "local_temperature",
            "occupied_heating_setpoint",
            "occupied_cooling_setpoint",
            "system_mode",
            "running_state",
            "keypad_lockout",
            "temperature_display_mode",
            "programming_operation_mode",
        ];
    }
    return generated;
}
function extenderFanControl(device, endpoints) {
    const generated = [];
    for (const endpoint of endpoints) {
        // If hvacThermostat is on the same endpoint, fanMode is handled by the thermostat extender
        if (endpoint.supportsInputCluster("hvacThermostat")) {
            continue;
        }
        let endpointNames;
        if (!onlyFirstDeviceEnpoint(device, endpoints)) {
            endpointNames = [endpoint.ID.toString()];
        }
        generated.push(new ExtendGenerator({ extend: m.fanControl, args: { endpointNames }, source: "fanControl" }));
    }
    return generated;
}
function extenderThermostatUi(device, endpoints) {
    const generated = [];
    for (const endpoint of endpoints) {
        let endpointNames;
        if (!onlyFirstDeviceEnpoint(device, endpoints)) {
            endpointNames = [endpoint.ID.toString()];
        }
        generated.push(new ExtendGenerator({ extend: m.thermostatUi, args: { endpointNames }, source: "thermostatUi" }));
    }
    return generated;
}
async function extenderBinaryInput(device, endpoints) {
    let endpointName;
    const labels = [];
    const descriptions = [];
    const homeassistants = [];
    const name = "binary_input";
    if (!onlyFirstDeviceEnpoint(device, endpoints)) {
        endpointName = endpoints.map((e) => e.ID.toString());
    }
    for (const endpoint of endpoints) {
        const label = await (0, utils_1.getClusterAttributeValue)(endpoint, "genBinaryInput", "description", undefined);
        let labelHomeAssistant;
        if (label) {
            labelHomeAssistant = { name: label };
        }
        const description = `Binary Input ${label ?? name} on endpoint ${endpoint.ID}`;
        labels.push(label);
        descriptions.push(description);
        homeassistants.push(labelHomeAssistant);
    }
    const args = {
        name: name,
        label: labels,
        cluster: "genBinaryInput",
        attribute: "presentValue",
        reporting: { min: "MIN", max: "MAX", change: 1 },
        valueOn: ["ON", 1],
        valueOff: ["OFF", 0],
        description: descriptions,
        access: "STATE_GET",
        endpointName,
        homeassistant: homeassistants,
    };
    return [new ExtendGenerator({ extend: m.binary, args, source: "binary" })];
}
async function extenderBinaryOutput(device, endpoints) {
    let endpointName;
    const labels = [];
    const descriptions = [];
    const homeassistants = [];
    const name = "binary_output";
    if (!onlyFirstDeviceEnpoint(device, endpoints)) {
        endpointName = endpoints.map((e) => e.ID.toString());
    }
    for (const endpoint of endpoints) {
        const label = await (0, utils_1.getClusterAttributeValue)(endpoint, "genBinaryOutput", "description", undefined);
        let labelHomeAssistant;
        if (label) {
            labelHomeAssistant = { name: label };
        }
        const description = `Binary Output ${label ?? name} on endpoint ${endpoint.ID}`;
        labels.push(label);
        descriptions.push(description);
        homeassistants.push(labelHomeAssistant);
    }
    const args = {
        name,
        label: labels,
        cluster: "genBinaryOutput",
        attribute: "presentValue",
        reporting: { min: "MIN", max: "MAX", change: 1 },
        valueOn: ["ON", 1],
        valueOff: ["OFF", 0],
        description: descriptions,
        access: "ALL",
        endpointName,
        homeassistant: homeassistants,
    };
    return [new ExtendGenerator({ extend: m.binary, args, source: "binary" })];
}
async function extenderAnalogInput(device, endpoints) {
    let endpointNames;
    const labels = [];
    const descriptions = [];
    const homeassistants = [];
    const names = [];
    const units = [];
    const valueMins = [];
    const valueMaxs = [];
    const valueSteps = [];
    const precisions = [];
    if (!onlyFirstDeviceEnpoint(device, endpoints)) {
        endpointNames = endpoints.map((e) => e.ID.toString());
    }
    for (const endpoint of endpoints) {
        const label = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogInput", "description", undefined);
        let labelHomeAssistant;
        if (label) {
            labelHomeAssistant = { name: label };
        }
        const applicationType = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogInput", "applicationType", undefined);
        let unit;
        let name;
        if (applicationType !== undefined) {
            const typeBits = (applicationType >> 16) & 0xff;
            name = consts_1.APPTYPE_NAME[typeBits];
            unit = consts_1.APPTYPE_UNIT[typeBits];
        }
        name = name ? `analog_in_${name}` : "analog_input";
        if (unit === undefined) {
            const bacnet_unit = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogInput", "engineeringUnits", undefined);
            if (bacnet_unit !== undefined) {
                unit = consts_1.BACNET_UNIT[bacnet_unit];
            }
        }
        const description = `Analog Input ${label ?? name} on endpoint ${endpoint.ID}`;
        let valueMin = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogInput", "minPresentValue", undefined);
        let valueMax = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogInput", "maxPresentValue", undefined);
        let valueStep = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogInput", "resolution", undefined);
        let precision;
        if (valueMin && !Number.isFinite(valueMin))
            valueMin = undefined;
        if (valueMax && !Number.isFinite(valueMax))
            valueMax = undefined;
        if (valueStep && Number.isFinite(valueStep) && valueStep > 0) {
            precision = (0, utils_1.resolutionToDecimalPrecision)(valueStep);
        }
        else {
            valueStep = undefined;
        }
        labels.push(label);
        descriptions.push(description);
        homeassistants.push(labelHomeAssistant);
        names.push(name);
        units.push(unit);
        valueMins.push(valueMin);
        valueMaxs.push(valueMax);
        valueSteps.push(valueStep);
        precisions.push(precision);
    }
    const args = {
        name: names,
        label: labels,
        valueMin: valueMins,
        valueMax: valueMaxs,
        valueStep: valueSteps,
        cluster: "genAnalogInput",
        attribute: "presentValue",
        reporting: { min: "MIN", max: "MAX", change: 1 },
        description: descriptions,
        access: "STATE_GET",
        endpointNames: endpointNames,
        unit: units,
        homeassistant: homeassistants,
        precision: precisions,
    };
    return [new ExtendGenerator({ extend: m.numeric, args, source: "numeric" })];
}
async function extenderAnalogOutput(device, endpoints) {
    let endpointNames;
    const labels = [];
    const descriptions = [];
    const homeassistants = [];
    const names = [];
    const units = [];
    const valueMins = [];
    const valueMaxs = [];
    const valueSteps = [];
    const precisions = [];
    if (!onlyFirstDeviceEnpoint(device, endpoints)) {
        endpointNames = endpoints.map((e) => e.ID.toString());
    }
    for (const endpoint of endpoints) {
        const label = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogOutput", "description", undefined);
        let labelHomeAssistant;
        if (label) {
            labelHomeAssistant = { name: label };
        }
        const applicationType = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogOutput", "applicationType", undefined);
        let unit;
        let name;
        if (applicationType !== undefined) {
            const typeBits = (applicationType >> 16) & 0xff;
            name = consts_1.APPTYPE_NAME[typeBits];
            unit = consts_1.APPTYPE_UNIT[typeBits];
        }
        name = name ? `analog_out_${name}` : "analog_output";
        if (unit === undefined) {
            const bacnet_unit = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogOutput", "engineeringUnits", undefined);
            if (bacnet_unit !== undefined) {
                unit = consts_1.BACNET_UNIT[bacnet_unit];
            }
        }
        const description = `Analog Output ${label ?? name} on endpoint ${endpoint.ID}`;
        let valueMin = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogOutput", "minPresentValue", undefined);
        let valueMax = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogOutput", "maxPresentValue", undefined);
        let valueStep = await (0, utils_1.getClusterAttributeValue)(endpoint, "genAnalogOutput", "resolution", undefined);
        let precision;
        if (valueMin && !Number.isFinite(valueMin))
            valueMin = undefined;
        if (valueMax && !Number.isFinite(valueMax))
            valueMax = undefined;
        if (valueStep && Number.isFinite(valueStep) && valueStep > 0) {
            precision = (0, utils_1.resolutionToDecimalPrecision)(valueStep);
        }
        else {
            valueStep = undefined;
        }
        labels.push(label);
        descriptions.push(description);
        homeassistants.push(labelHomeAssistant);
        names.push(name);
        units.push(unit);
        valueMins.push(valueMin);
        valueMaxs.push(valueMax);
        valueSteps.push(valueStep);
        precisions.push(precision);
    }
    const args = {
        name: names,
        label: labels,
        valueMin: valueMins,
        valueMax: valueMaxs,
        valueStep: valueSteps,
        cluster: "genAnalogOutput",
        attribute: "presentValue",
        reporting: { min: "MIN", max: "MAX", change: 1 },
        description: descriptions,
        access: "ALL",
        endpointNames: endpointNames,
        unit: units,
        homeassistant: homeassistants,
        precision: precisions,
    };
    return [new ExtendGenerator({ extend: m.numeric, args, source: "numeric" })];
}
//# sourceMappingURL=generateDefinition.js.map