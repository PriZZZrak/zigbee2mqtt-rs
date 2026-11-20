// Single implementation of the upstream converter call path used by both the
// single-call benchmark (runner.mjs) and the concurrency/stress benchmark
// (stress.mjs). Mocks only replace the Zigbee/host boundary.

import { createDeviceAndEndpoint, normalizeValue } from "./mocks.mjs";

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

export function makeLogger() {
  const noop = () => {};
  return { debug: noop, info: noop, warn: noop, error: noop };
}

function makeMeta(scenario, device, definition, state, options, published) {
  return {
    device,
    logger: makeLogger(),
    state: clone(state ?? {}),
    deviceExposesChanged: () => {},
    endpoint_name: scenario.endpointName,
    options: clone(options ?? {}),
    message: clone(scenario.message ?? {}),
    membersState: {},
    mapped: definition,
    publish: async (payload) => published.push(normalizeValue(payload)),
  };
}

export function makeEndpoint(scenario, host, zcl) {
  const { device, endpoint } = createDeviceAndEndpoint(
    {
      ...scenario,
      device: scenario.device
        ? {
            ...scenario.device,
            endpoints:
              scenario.device.endpoints ??
              [{ id: 1, inputClusters: [], outputClusters: [] }],
          }
        : { endpoints: [{ id: 1, inputClusters: [], outputClusters: [] }] },
    },
    host,
    zcl,
  );
  // Device constructor in mocks creates endpoints; ensure the selected one is
  // present even for endpointId=2.
  return { device, endpoint };
}

export async function invokeConverter({
  upstream,
  scenario,
  prepared,
  host,
  device,
  endpoint,
  iteration,
}) {
  const definition = prepared.definition;
  // Converter-only span: upstream converter callbacks, excluding harness
  // context reset, message/meta construction and result normalization.
  let converterMs = 0;
  const timeConverter = async (fn) => {
    const started = performance.now();
    try {
      return await fn();
    } finally {
      converterMs += performance.now() - started;
    }
  };

  const state = { ...(scenario.state ?? {}) };
  const options = { ...(scenario.options ?? {}) };
  const published = [];
  const meta = makeMeta(scenario, device, definition, state, options, published);

  if (scenario.kind === "fromZigbee") {
    const type = scenario.msg.type ?? "attributeReport";
    const converters = definition.fromZigbee.filter((candidate) => {
      const types = Array.isArray(candidate.type) ? candidate.type : [candidate.type];
      return candidate.cluster === scenario.msg.cluster && types.includes(type);
    });
    if (converters.length === 0) {
      throw new Error(
        `no fromZigbee converter for ${definition.model}:${scenario.msg.cluster}/${type}`,
      );
    }
    const msg = {
      ...clone(scenario.msg),
      device,
      endpoint,
      meta: {
        ...(scenario.msg.meta ?? {}),
        zclTransactionSequenceNumber:
          (scenario.msg.meta?.zclTransactionSequenceNumber ?? 1) + iteration,
      },
    };
    const publish = async (payload) => {
      upstream.pkg.postProcessConvertedFromZigbeeMessage(definition, payload, options, device);
      published.push(normalizeValue(payload));
    };
    const convertMeta = {
      device,
      logger: makeLogger(),
      state: clone(state),
      deviceExposesChanged: () => {},
    };
    let merged = {};
    for (const converter of converters) {
      const converted = await timeConverter(() =>
        converter.convert(definition, msg, publish, options, convertMeta),
      );
      if (converted) merged = { ...merged, ...converted };
    }
    if (Object.keys(merged).length > 0) await publish(merged);
    return {
      result: normalizeValue(Object.keys(merged).length > 0 ? merged : null),
      published,
      calls: host.calls,
      waitMs: host.waitMs,
      converterMs,
    };
  }

  if (scenario.kind === "set" || scenario.kind === "get") {
    const converters = definition.toZigbee.filter((candidate) => {
      const keys = Array.isArray(candidate.key) ? candidate.key : [candidate.key];
      return keys.includes(scenario.key);
    });
    const converter =
      converters.find((candidate) => candidate.key !== undefined) ??
      definition.toZigbee.find((candidate) => candidate.key === undefined);
    if (!converter) {
      throw new Error(`no toZigbee converter for ${definition.model}:${scenario.key}`);
    }
    const result = await timeConverter(() =>
      scenario.kind === "set"
        ? converter.convertSet(endpoint, scenario.key, scenario.value, meta)
        : converter.convertGet(endpoint, scenario.key, meta),
    );
    return {
      result: normalizeValue(result ?? null),
      published,
      calls: host.calls,
      waitMs: host.waitMs,
      converterMs,
    };
  }

  if (scenario.kind === "configure") {
    if (!definition.configure) throw new Error(`no configure for ${definition.model}`);
    const coordinatorDevice = {
      ...device,
      ieeeAddr: "0x00124b0000000000",
      networkAddress: 0,
    };
    const coordinatorEndpoint = makeEndpoint(
      {
        ...scenario,
        device: { endpoints: [{ id: 1, inputClusters: [], outputClusters: [] }] },
      },
      host,
      upstream.zcl(),
    ).endpoint;
    coordinatorEndpoint.deviceIeeeAddress = coordinatorDevice.ieeeAddr;
    await timeConverter(() => definition.configure(device, coordinatorEndpoint, definition));
    return { result: null, published, calls: host.calls, waitMs: host.waitMs, converterMs };
  }

  if (scenario.kind === "onEvent") {
    if (!definition.onEvent) throw new Error(`no onEvent for ${definition.model}`);
    const event = {
      type: scenario.eventType,
      data: {
        device,
        state: clone(state),
        options: clone(options),
        deviceExposesChanged: () => {},
      },
    };
    if (scenario.eventType === "deviceInterview") event.data.status = "successful";
    await timeConverter(() => definition.onEvent(event));
    return { result: null, published, calls: host.calls, waitMs: host.waitMs, converterMs };
  }

  throw new Error(`unknown scenario kind ${scenario.kind}`);
}
