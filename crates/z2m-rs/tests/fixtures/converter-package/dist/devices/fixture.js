"use strict";

const fzOnOff = {
  cluster: "genOnOff",
  type: ["attributeReport", "readResponse"],
  convert: async function (model, msg) {
    const value = msg.data.onOff;
    return { state: value ? "ON" : "OFF", [`${msg.device.ieeeAddr}_last`]: value ? "ON" : "OFF" };
  },
};

const tzState = {
  key: ["state"],
  convertSet: async function (entity, key, value) {
    await entity.command("genOnOff", String(value).toLowerCase(), {}, {});
    return { state: { state: String(value).toUpperCase() } };
  },
  convertGet: async function (entity) {
    const result = await entity.read("genOnOff", ["onOff"]);
    return { state: { state: result.onOff ? "ON" : "OFF" } };
  },
};

exports.definitions = [
  {
    model: "FixtureLamp",
    vendor: "Fixture",
    description: "Offline bridge fixture lamp",
    zigbeeModel: ["FixtureLamp"],
    exposes: [],
    fromZigbee: [fzOnOff],
    toZigbee: [tzState],
    configure: async function (device, coordinatorEndpoint) {
      await coordinatorEndpoint.read("genOnOff", ["onOff"]);
      await coordinatorEndpoint.write("genOnOff", { onOff: true });
      await new Promise(function (resolve) { setTimeout(resolve, 50); });
    },
    onEvent: async function (event) {
      if (event.type === "start") {
        await event.data.device.endpoints[0].read("genOnOff", ["onOff"]);
      }
      if (event.type === "deviceJoined") {
        await new Promise(function (resolve) { setTimeout(resolve, 10000); });
      }
    },
  },
  {
    model: "FixtureSwitch",
    vendor: "Fixture",
    description: "Offline bridge fixture switch",
    zigbeeModel: ["FixtureSwitch"],
    exposes: [],
    fromZigbee: [fzOnOff],
    toZigbee: [tzState],
    configure: async function (device, coordinatorEndpoint) {
      await coordinatorEndpoint.read("genOnOff", ["onOff"]);
    },
  },
];
