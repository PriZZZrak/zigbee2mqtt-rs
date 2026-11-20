// Benchmark scenarios. All definitions and converters are real upstream code.
//
// Each scenario is a small, deterministic input for the same converter call
// path used by zigbee2mqtt. The expected result and host calls are recorded
// into the golden corpus on the first run.

const broadClusters = [0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 15, 20, 25, 32, 513, 516, 1024, 1026, 1029, 1280, 1794, 2820, 2821, 61184, 64512, 64640];

function device(model, extra = {}) {
  return {
    modelID: model,
    manufacturerName: extra.manufacturerName ?? "Bench",
    manufacturerID: extra.manufacturerID ?? 0x1234,
    type: extra.type ?? "Router",
    ieeeAddr: extra.ieeeAddr ?? "0x00124b0000000001",
    networkAddress: extra.networkAddress ?? 0x1234,
    endpoints: extra.endpoints ?? [
      { id: 1, inputClusters: broadClusters, outputClusters: [] },
    ],
  };
}

function fz(id, definition, message, extra = {}) {
  return {
    id,
    kind: "fromZigbee",
    definition,
    device: device(extra.model ?? definition.model, extra.device),
    endpointId: extra.endpointId ?? 1,
    msg: {
      cluster: message.cluster,
      type: message.type ?? "attributeReport",
      data: message.data,
      linkquality: message.linkquality ?? 100,
      groupID: message.groupID,
      meta: {
        zclTransactionSequenceNumber: message.tsn ?? 1,
        manufacturerCode: message.manufacturerCode,
      },
    },
    options: extra.options ?? {},
    state: extra.state ?? {},
    meta: extra.meta ?? {},
    iterations: extra.iterations,
    hostDelayMs: extra.hostDelayMs,
    readValues: extra.readValues,
  };
}

function set(id, definition, key, value, extra = {}) {
  return {
    id,
    kind: "set",
    definition,
    device: device(extra.model ?? definition.model, extra.device),
    endpointId: extra.endpointId ?? 1,
    key,
    value,
    message: extra.message ?? { [key]: value },
    endpointName: extra.endpointName,
    options: extra.options ?? {},
    state: extra.state ?? {},
    iterations: extra.iterations,
    hostDelayMs: extra.hostDelayMs,
    readValues: extra.readValues,
  };
}

function get(id, definition, key, extra = {}) {
  return {
    id,
    kind: "get",
    definition,
    device: device(extra.model ?? definition.model, extra.device),
    endpointId: extra.endpointId ?? 1,
    key,
    message: extra.message ?? {},
    endpointName: extra.endpointName,
    options: extra.options ?? {},
    state: extra.state ?? {},
    iterations: extra.iterations,
    hostDelayMs: extra.hostDelayMs,
    readValues: extra.readValues,
  };
}

function configure(id, definition, extra = {}) {
  return {
    id,
    kind: "configure",
    definition,
    device: device(extra.model ?? definition.model, extra.device),
    endpointId: extra.endpointId ?? 1,
    options: extra.options ?? {},
    state: extra.state ?? {},
    iterations: extra.iterations ?? 10,
    hostDelayMs: extra.hostDelayMs,
    readValues: extra.readValues,
  };
}

function onEvent(id, definition, eventType, extra = {}) {
  return {
    id,
    kind: "onEvent",
    definition,
    device: device(extra.model ?? definition.model, extra.device),
    endpointId: extra.endpointId ?? 1,
    eventType,
    options: extra.options ?? {},
    state: extra.state ?? {},
    iterations: extra.iterations ?? 20,
    hostDelayMs: extra.hostDelayMs,
    readValues: extra.readValues,
  };
}

const ikea = { source: "package", file: "ikea.js", model: "LED1545G12" };
const lumi = { source: "package", file: "lumi.js", model: "RTCGQ01LM" };
const lytko = { source: "package", file: "lytko.js", model: "L101Z-SBI" };
const avatto = { source: "package", file: "avatto.js", model: "ZWSH16" };
const tuya = { source: "package", file: "tuya.js", model: "TS0002" };
const danfoss = { source: "package", file: "danfoss.js", model: "014G2461" };
const daewoo = { source: "package", file: "daewoo.js", model: "WKE502Z" };
const external = { source: "external", model: "L101Ze.v2" };
const m9pro = { source: "package", file: "tuya.js", model: "M9Pro" };
const yokis = { source: "package", file: "yokis.js", model: "MTV300E-UP" };
const icon2 = { source: "package", file: "danfoss.js", model: "Icon2" };

export const scenarios = [
  {
    id: "package-load",
    kind: "package",
    iterations: 1,
    description: "Cold require of the converter package root and zigbee-herdsman.",
  },
  {
    id: "prepare-ikea-modern-extend",
    kind: "prepare",
    definition: ikea,
    iterations: 100,
    description: "prepareDefinition for a modernExtend light (merge extends/exposes/options).",
  },
  {
    id: "prepare-danfoss-thermostat",
    kind: "prepare",
    definition: danfoss,
    iterations: 50,
    description: "prepareDefinition for a thermostat with large exposes/options and onEvent.",
  },
  {
    id: "prepare-tuya-sensor",
    kind: "prepare",
    definition: avatto,
    iterations: 50,
    description: "prepareDefinition for a modernExtend Tuya datapoint device.",
  },
  fz("fz-ikea-onoff", ikea, { cluster: "genOnOff", data: { onOff: 1 } }),
  fz("fz-ikea-brightness", ikea, { cluster: "genLevelCtrl", data: { currentLevel: 128 } }),
  fz("fz-ikea-color-temp", ikea, { cluster: "lightingColorCtrl", data: { colorTemperature: 250 } }, {
    readValues: {
      "lightingColorCtrl:colorTempPhysicalMin": 220,
      "lightingColorCtrl:colorTempPhysicalMax": 400,
    },
  }),
  fz("fz-ikea-color-xy", ikea, {
    cluster: "lightingColorCtrl",
    data: { currentX: 0x616b, currentY: 0xed6a },
  }, {
    readValues: {
      "lightingColorCtrl:colorTempPhysicalMin": 220,
      "lightingColorCtrl:colorTempPhysicalMax": 400,
    },
  }),
  set("set-ikea-onoff", ikea, "state", "ON"),
  set("set-ikea-brightness", ikea, "brightness", 128),
  set("set-ikea-color-temp", ikea, "color_temp", 250, {
    readValues: {
      "lightingColorCtrl:colorTempPhysicalMin": 220,
      "lightingColorCtrl:colorTempPhysicalMax": 400,
    },
  }),
  get("get-ikea-onoff", ikea, "state"),
  set("set-ikea-brightness-host-delay", ikea, "brightness", 128, {
    hostDelayMs: 2,
    iterations: 20,
    description: "Same set path with a 2 ms host response, to separate JS from I/O wait.",
  }),
  fz("fz-lumi-occupancy", lumi, { cluster: "msOccupancySensing", data: { occupancy: 1 } }),
  fz("fz-lumi-occupancy-clear", lumi, { cluster: "msOccupancySensing", data: { occupancy: 0 } }),
  fz("fz-l101-thermostat", lytko, {
    cluster: "hvacThermostat",
    data: { localTemp: 2150, occupiedHeatingSetpoint: 2000 },
    manufacturerCode: 0x7777,
  }),
  set("set-l101-setpoint", lytko, "occupied_heating_setpoint", 21.5, {
    message: { occupied_heating_setpoint: 21.5 },
  }),
  get("get-l101-sensor-type", lytko, "sensor_type", { message: { sensor_type: "10 кОм" } }),
  fz("fz-tuya-dp-temperature", avatto, {
    cluster: "manuSpecificTuya",
    type: "commandDataReport",
    data: { dpValues: [{ dp: 1, datatype: 2, data: [0, 0, 0, 215] }] },
    tsn: 11,
  }),
  fz("fz-tuya-dp-multi", avatto, {
    cluster: "manuSpecificTuya",
    type: "commandDataResponse",
    data: {
      dpValues: [
        { dp: 1, datatype: 2, data: [0, 0, 0, 220] },
        { dp: 2, datatype: 2, data: [0, 0, 0, 44] },
        { dp: 4, datatype: 2, data: [0, 0, 0, 88] },
      ],
    },
    tsn: 12,
  }),
  set("set-tuya-temperature-unit", avatto, "temperature_unit", "fahrenheit", {
    message: { temperature_unit: "fahrenheit" },
  }),
  fz("fz-ts0002-ep2-onoff", tuya, {
    cluster: "genOnOff",
    data: { onOff: 1 },
    endpointId: 2,
  }, {
    device: {
      endpoints: [
        { id: 1, inputClusters: broadClusters, outputClusters: [] },
        { id: 2, inputClusters: broadClusters, outputClusters: [] },
      ],
    },
    endpointId: 2,
  }),
  set("set-ts0002-ep2-onoff", tuya, "state", "ON", {
    endpointName: "l2",
    endpointId: 2,
    message: { state: "ON", state_l2: "ON" },
    device: {
      endpoints: [
        { id: 1, inputClusters: broadClusters, outputClusters: [] },
        { id: 2, inputClusters: broadClusters, outputClusters: [] },
      ],
    },
  }),
  fz("fz-danfoss-temperature", danfoss, {
    cluster: "hvacThermostat",
    data: { localTemp: 2100, occupiedHeatingSetpoint: 2000 },
    tsn: 21,
  }),
  get("get-danfoss-schedule", danfoss, "weekly_schedule", {
    message: { weekly_schedule: {} },
  }),
  configure("configure-l101z", lytko, {
    iterations: 50,
    device: {
      endpoints: [
        { id: 1, inputClusters: broadClusters, outputClusters: [] },
        { id: 2, inputClusters: broadClusters, outputClusters: [] },
        { id: 3, inputClusters: broadClusters, outputClusters: [] },
      ],
    },
  }),
  configure("configure-danfoss", danfoss, { iterations: 50 }),
  onEvent("onEvent-daewoo-start", daewoo, "start", { iterations: 50 }),
  onEvent("onEvent-daewoo-announce", daewoo, "deviceAnnounce", { iterations: 50 }),
  {
    id: "prepare-external-l101",
    kind: "prepare",
    definition: external,
    iterations: 50,
    description: "prepareDefinition for the real L101Ze_z2m.js external converter.",
  },
  fz("fz-external-l101-temp", external, {
    cluster: "hvacThermostat",
    data: { localTemp: 2150, occupiedHeatingSetpoint: 2000 },
    manufacturerCode: 0x7777,
    tsn: 31,
  }),
  set("set-external-l101-mode", external, "system_mode", "heat", {
    message: { system_mode: "heat" },
  }),
  get("get-external-l101-sensor", external, "sensor_type", {
    message: { sensor_type: "10 кОм" },
  }),
  configure("configure-external-l101", external, { iterations: 30 }),
  {
    id: "prepare-heavy-yokis-81-extends",
    kind: "prepare",
    definition: yokis,
    iterations: 10,
    description: "Top static-complexity definition from heavy-scan.mjs (81 extends).",
  },
  {
    id: "prepare-heavy-icon2-thermostat",
    kind: "prepare",
    definition: icon2,
    iterations: 10,
    description: "Heavy thermostat definition: 9 fromZigbee, 22 toZigbee, configure.",
  },
  fz("fz-tuya-m9pro-multi-dp", m9pro, {
    cluster: "manuSpecificTuya",
    type: "commandDataReport",
    data: {
      dpValues: [
        { dp: 1, datatype: 4, data: [1] },
        { dp: 2, datatype: 4, data: [2] },
        { dp: 18, datatype: 4, data: [3] },
        { dp: 20, datatype: 4, data: [3] },
      ],
    },
    tsn: 41,
  }),
];
