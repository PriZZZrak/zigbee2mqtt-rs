// Deterministic upstream-converter mocks for the Node benchmark.
//
// The classes intentionally mirror the observable `zigbee-herdsman`
// Device/Endpoint/Group surface used by zigbee-herdsman-converters. Every host
// operation is recorded so the same input can be replayed against a future
// Rust runtime.

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

export class MockHost {
  constructor({ delayMs = 0, readValues = {}, zcl = null } = {}) {
    this.calls = [];
    this.delayMs = delayMs;
    this.readValues = readValues;
    this.zcl = zcl;
    this.waitMs = 0;
  }

  record(op) {
    this.calls.push(op);
    return op;
  }

  async delay() {
    if (this.delayMs > 0) {
      const started = performance.now();
      await new Promise((resolve) => setTimeout(resolve, this.delayMs));
      this.waitMs += performance.now() - started;
    }
  }

  readResponse(cluster, attributes) {
    const result = {};
    for (const attribute of attributes ?? []) {
      const key = `${cluster}:${attribute}`;
      result[attribute] = this.readValues[key] ?? this.readValues[attribute] ?? 0;
    }
    return result;
  }
}

export class Endpoint {
  constructor({
    id = 1,
    device,
    inputClusters = [],
    outputClusters = [],
    meta = {},
    host,
  }) {
    this.ID = id;
    this.deviceIeeeAddress = device.ieeeAddr;
    this.deviceNetworkAddress = device.networkAddress;
    this.inputClusters = inputClusters;
    this.outputClusters = outputClusters;
    this.meta = meta;
    this.binds = [];
    this.configuredReportings = [];
    this.host = host;
    this.device = device;
  }

  async read(cluster, attributes, options) {
    const call = this.host.record({
      op: "read",
      endpoint: this.ID,
      cluster: String(cluster),
      attributes: [...(attributes ?? [])],
      options: options ?? {},
    });
    await this.host.delay();
    return this.host.readResponse(String(cluster), call.attributes);
  }

  async write(cluster, attributes, options) {
    this.host.record({
      op: "write",
      endpoint: this.ID,
      cluster: String(cluster),
      attributes: normalizeAttributes(attributes),
      options: options ?? {},
    });
    await this.host.delay();
  }

  async command(cluster, command, payload, options) {
    this.host.record({
      op: "command",
      endpoint: this.ID,
      cluster: String(cluster),
      command: String(command),
      payload: normalizeValue(payload),
      options: options ?? {},
    });
    await this.host.delay();
  }

  async commandResponse(cluster, command, payload, options) {
    this.host.record({
      op: "commandResponse",
      endpoint: this.ID,
      cluster: String(cluster),
      command: String(command),
      payload: normalizeValue(payload),
      options: options ?? {},
    });
    await this.host.delay();
    return { status: 0 };
  }

  async bind(cluster, target, options) {
    this.host.record({
      op: "bind",
      endpoint: this.ID,
      cluster: String(cluster),
      target: target?.deviceIeeeAddress
        ? { type: "endpoint", ieeeAddr: target.deviceIeeeAddress, endpoint: target.ID }
        : { type: "group", groupID: target?.groupID },
      options: options ?? {},
    });
    this.binds.push({ cluster, target });
    await this.host.delay();
  }

  async configureReporting(cluster, items, options) {
    this.host.record({
      op: "configureReporting",
      endpoint: this.ID,
      cluster: String(cluster),
      items: normalizeValue(items),
      options: options ?? {},
    });
    this.configuredReportings.push({ cluster, items });
    await this.host.delay();
  }

  getClusterAttributeValue(cluster, attribute) {
    return this.host.readValues[`${cluster}:${attribute}`];
  }

  getCluster(clusterKey, manufacturerCode) {
    if (this.host.zcl?.Utils?.getCluster) {
      try {
        return this.host.zcl.Utils.getCluster(clusterKey, manufacturerCode, this.device.customClusters ?? {});
      } catch {
        // Custom/unknown cluster: return a minimal shape.
      }
    }
    return { ID: typeof clusterKey === "number" ? clusterKey : 0, name: String(clusterKey), attributes: {}, commands: {} };
  }

  getDevice() {
    return this.device;
  }

  getInputClusters() {
    return this.inputClusters.map((cluster) => this.getCluster(cluster));
  }

  getOutputClusters() {
    return this.outputClusters.map((cluster) => this.getCluster(cluster));
  }

  clusterName(cluster) {
    return typeof cluster === "number" ? this.getCluster(cluster).name : String(cluster);
  }

  supportsInputCluster(cluster) {
    const wanted = this.clusterName(cluster);
    return this.inputClusters.some((candidate) => this.clusterName(candidate) === wanted);
  }

  supportsOutputCluster(cluster) {
    const wanted = this.clusterName(cluster);
    return this.outputClusters.some((candidate) => this.clusterName(candidate) === wanted);
  }

  toString() {
    return `endpoint ${this.ID}`;
  }
}

export class Group {
  constructor({ groupID = 1, members = [], host }) {
    this.groupID = groupID;
    this.members = members;
    this.host = host;
    this.ID = groupID;
  }

  hasMember(endpoint) {
    return this.members.some(
      (member) => member.deviceIeeeAddress === endpoint.deviceIeeeAddress && member.ID === endpoint.ID,
    );
  }

  async command() {
    this.host.record({ op: "groupCommand", groupID: this.groupID });
  }

  toString() {
    return `group ${this.groupID}`;
  }
}

export class Device {
  constructor({
    ieeeAddr = "0x00124b0000000001",
    networkAddress = 0x1234,
    modelID = "TEST",
    manufacturerID = 0x1234,
    manufacturerName = "Test",
    powerSource = "Mains (single phase)",
    type = "Router",
    softwareBuildID = "1.0.0",
    dateCode = "20260101",
    endpoints = [{ id: 1, inputClusters: [], outputClusters: [] }],
    host,
  }) {
    this.ieeeAddr = ieeeAddr;
    this.networkAddress = networkAddress;
    this.modelID = modelID;
    this.manufacturerID = manufacturerID;
    this.manufacturerName = manufacturerName;
    this.powerSource = powerSource;
    this.type = type;
    this.softwareBuildID = softwareBuildID;
    this.dateCode = dateCode;
    this.zclVersion = 3;
    this.appVersion = 1;
    this.stackVersion = 2;
    this.hwVersion = 1;
    this.meta = {};
    this.customClusters = {};
    this.customReadResponse = undefined;
    this.genBasic = {
      modelId: modelID,
      manufacturerName,
      powerSource: 1,
      zclVersion: 3,
      appVersion: 1,
      stackVersion: 2,
      hwVersion: 1,
      dateCode,
      swBuildId: softwareBuildID,
    };
    this.host = host;
    this.endpoints = endpoints.map((endpoint) => new Endpoint({ ...endpoint, device: this, host }));
    for (const endpoint of this.endpoints) {
      endpoint.device = this;
    }
  }

  getEndpoint(id) {
    return this.endpoints.find((endpoint) => endpoint.ID === id);
  }

  addCustomCluster(name, definition) {
    this.customClusters[name] = definition;
    this.host.record({ op: "addCustomCluster", name });
  }

  save() {
    this.host.record({ op: "save", ieeeAddr: this.ieeeAddr });
  }
}

export function createDeviceAndEndpoint(scenario, host, zcl) {
  const device = new Device({ ...(scenario.device ?? {}), host });
  if (scenario.device?.endpoints) {
    // Device constructor already built endpoints; replace with the requested set.
    device.endpoints = scenario.device.endpoints.map(
      (endpoint) => new Endpoint({ ...endpoint, device, host }),
    );
  }
  const endpointId = scenario.endpointId ?? device.endpoints[0]?.ID ?? 1;
  const endpoint = device.getEndpoint(endpointId) ?? device.endpoints[0];
  return { device, endpoint, host };
}

export function normalizeValue(value) {
  if (value === undefined) return null;
  if (typeof value === "bigint") return value.toString();
  if (value instanceof Uint8Array) return [...value];
  if (Array.isArray(value)) return value.map(normalizeValue);
  if (value && typeof value === "object") {
    const result = {};
    for (const [key, item] of Object.entries(value)) result[key] = normalizeValue(item);
    return result;
  }
  return value;
}

function normalizeAttributes(attributes) {
  if (!attributes) return {};
  const result = {};
  for (const [key, value] of Object.entries(attributes)) {
    result[key] = normalizeValue(value);
  }
  return result;
}
