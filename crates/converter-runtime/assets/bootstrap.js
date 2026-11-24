"use strict";
// Minimal CommonJS loader + host shims for the engine feasibility study.
// This is NOT a production runtime: it exists to execute the real upstream
// converter package inside a candidate engine and expose the same scenario
// shapes as tools/bench/runner.mjs.

(function () {
  var root = globalThis.__root;
  var externalPath = globalThis.__external;
  var zclPath = globalThis.__zclPath;
  var herdsmanRoot = globalThis.__herdsmanRoot;
  var moduleCache = Object.create(null);
  var modelsIndex = null;
  var definitionCache = Object.create(null);
  var preparedCache = Object.create(null);

  function dirname(path) {
    var index = path.lastIndexOf("/");
    return index <= 0 ? "/" : path.slice(0, index);
  }
  function basename(path) {
    var index = path.lastIndexOf("/");
    return index < 0 ? path : path.slice(index + 1);
  }
  function normalizePath(path) {
    var absolute = path.charAt(0) === "/";
    var parts = path.split("/");
    var out = [];
    for (var i = 0; i < parts.length; i++) {
      var part = parts[i];
      if (!part || part === ".") continue;
      if (part === "..") { if (out.length) out.pop(); continue; }
      out.push(part);
    }
    return (absolute ? "/" : "") + out.join("/");
  }
  function join() {
    var parts = [];
    for (var i = 0; i < arguments.length; i++) {
      if (arguments[i]) parts.push(arguments[i]);
    }
    return normalizePath(parts.join("/"));
  }
  function exists(path) {
    try {
      return __fileExists(path);
    } catch (error) {
      return false;
    }
  }
  function resolveFile(base) {
    var candidates = [base, base + ".js", base + ".json", join(base, "index.js")];
    for (var i = 0; i < candidates.length; i++) {
      if (exists(candidates[i])) return candidates[i];
    }
    return null;
  }

  // ---- tiny Buffer -------------------------------------------------------
  class Buffer extends Uint8Array {
    static from(value, encoding) {
      if (typeof value === "string") return new Buffer(encodeUtf8(value));
      if (value instanceof Uint8Array) return new Buffer(value);
      if (Array.isArray(value)) return new Buffer(value);
      if (value && value.type === "Buffer" && Array.isArray(value.data)) {
        return new Buffer(value.data);
      }
      throw new Error("Buffer.from: unsupported input");
    }
    static alloc(size) {
      return new Buffer(size);
    }
    static isBuffer(value) {
      return value instanceof Buffer;
    }
    toString(encoding) {
      return decodeUtf8(this);
    }
    readUIntBE(offset, length) {
      var value = 0;
      for (var i = 0; i < length; i++) value = value * 256 + this[offset + i];
      return value;
    }
  }

  function encodeUtf8(text) {
    var bytes = [];
    for (var i = 0; i < text.length; i++) {
      var code = text.charCodeAt(i);
      if (code < 0x80) bytes.push(code);
      else if (code < 0x800) bytes.push(0xc0 | (code >> 6), 0x80 | (code & 63));
      else if (code >= 0xd800 && code < 0xdc00) {
        var next = text.charCodeAt(++i);
        var cp = 0x10000 + ((code - 0xd800) << 10) + (next - 0xdc00);
        bytes.push(0xf0 | (cp >> 18), 0x80 | ((cp >> 12) & 63), 0x80 | ((cp >> 6) & 63), 0x80 | (cp & 63));
      } else bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 63), 0x80 | (code & 63));
    }
    return bytes;
  }
  function decodeUtf8(bytes) {
    var out = "";
    for (var i = 0; i < bytes.length; ) {
      var b = bytes[i++];
      if (b < 0x80) out += String.fromCharCode(b);
      else if (b < 0xe0) out += String.fromCharCode(((b & 31) << 6) | (bytes[i++] & 63));
      else if (b < 0xf0) out += String.fromCharCode(((b & 15) << 12) | ((bytes[i++] & 63) << 6) | (bytes[i++] & 63));
      else {
        var cp = ((b & 7) << 18) | ((bytes[i++] & 63) << 12) | ((bytes[i++] & 63) << 6) | (bytes[i++] & 63);
        cp -= 0x10000;
        out += String.fromCharCode(0xd800 + (cp >> 10), 0xdc00 + (cp & 1023));
      }
    }
    return out;
  }

  var TextEncoderImpl = typeof TextEncoder !== "undefined" ? TextEncoder : class {
    encode(text) { return new Uint8Array(encodeUtf8(text)); }
  };
  var TextDecoderImpl = typeof TextDecoder !== "undefined" ? TextDecoder : class {
    decode(bytes) { return decodeUtf8(bytes); }
  };

  // ---- Zcl shim from a JSON snapshot of zigbee-herdsman ------------------
  var zcl = JSON.parse(globalThis.__zclJson || __readFile(zclPath));
  var clusterCache = Object.create(null);
  zcl.Utils = {
    getCluster: function (name, manufacturerCode, customClusters) {
      perf.getCluster = (perf.getCluster || 0) + 1;
      if (!manufacturerCode && !customClusters && clusterCache[name]) return clusterCache[name];
      if (customClusters && customClusters[name]) return customClusters[name];
      var cluster = zcl.Clusters[name];
      if (!cluster) {
        var wanted = typeof name === "number" ? name : Number(name);
        for (var key in zcl.Clusters) {
          if (zcl.Clusters[key].ID === wanted) { cluster = zcl.Clusters[key]; break; }
        }
      }
      if (!cluster) {
        var fallbackId = Number(name);
        if (!Number.isNaN(fallbackId)) {
          return { name: String(name), ID: fallbackId, attributes: {}, commands: {}, commandsResponse: {} };
        }
        throw new Error("Unknown cluster " + String(name));
      }
      if (!manufacturerCode && !customClusters) clusterCache[name] = cluster;
      return cluster;
    },
    getClusterAttribute: function (clusterName, attributeName) {
      var cluster = zcl.Utils.getCluster(clusterName);
      var attribute = cluster.attributes[attributeName];
      if (!attribute) throw new Error("Unknown attribute " + clusterName + "." + attributeName);
      return attribute;
    },
  };

  // ---- builtins ----------------------------------------------------------
  function assert(value, message) {
    if (!value) throw new Error(message || "Assertion failed");
  }
  assert.ok = assert;
  assert.fail = function (message) { throw new Error(message || "Assertion failed"); };
  assert.strictEqual = function (a, b, message) {
    if (a !== b) throw new Error(message || "strictEqual failed");
  };
  assert.deepStrictEqual = function (a, b, message) {
    if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error(message || "deepStrictEqual failed");
  };
  assert.ifError = function (value) { if (value) throw value; };
  assert.default = assert;

  function parseVersion(value) {
    var match = /^(\d+)\.(\d+)\.(\d+)/.exec(String(value));
    return match ? [Number(match[1]), Number(match[2]), Number(match[3])] : null;
  }
  function compareVersion(a, b) {
    var left = parseVersion(a) || [0, 0, 0];
    var right = parseVersion(b) || [0, 0, 0];
    for (var i = 0; i < 3; i++) {
      if (left[i] !== right[i]) return left[i] < right[i] ? -1 : 1;
    }
    return 0;
  }
  var semver = {
    valid: function (value) { return parseVersion(value) ? String(value) : null; },
    gt: function (a, b) { return compareVersion(a, b) > 0; },
    gte: function (a, b) { return compareVersion(a, b) >= 0; },
    lt: function (a, b) { return compareVersion(a, b) < 0; },
    lte: function (a, b) { return compareVersion(a, b) <= 0; },
    satisfies: function (value, range) { return compareVersion(value, String(range).replace(/[^0-9.]/g, "")) >= 0; },
  };

  var pathShim = { join: join, dirname: dirname, basename: basename, extname: function (p) {
    var b = basename(p); var i = b.lastIndexOf("."); return i <= 0 ? "" : b.slice(i);
  }, resolve: function () { return join.apply(null, arguments); }, sep: "/" };

  var utilShim = {
    TextEncoder: TextEncoderImpl,
    TextDecoder: TextDecoderImpl,
    inspect: function (value) { try { return JSON.stringify(value); } catch (e) { return String(value); } },
    format: function () { return Array.prototype.join.call(arguments, " "); },
    promisify: function (fn) {
      return function () {
        var args = Array.prototype.slice.call(arguments);
        var self = this;
        return new Promise(function (resolve, reject) {
          args.push(function (error, value) { error ? reject(error) : resolve(value); });
          fn.apply(self, args);
        });
      };
    },
  };

  var zclModule = Object.assign({}, zcl, { Zcl: zcl });
  var builtins = {
    "zigbee-herdsman": { Zcl: zcl, ZSpec: {}, getTimeClusterAttributes: function () {
      return {
        time: Math.round((Date.now() - Date.UTC(2000, 0, 1)) / 1000),
        timeZone: -new Date().getTimezoneOffset() * 60,
        dstStart: 4294967295,
        dstEnd: 4294967295,
        dstShift: 0,
      };
    } },
    "zigbee-herdsman/dist/zspec/zcl": zclModule,
    "zigbee-herdsman/dist/zspec/zcl/definition/enums": zcl,
    "zigbee-herdsman/dist/zspec/zcl/definition/consts": zcl.Consts,
    "node:assert": assert,
    assert: assert,
    "node:buffer": { Buffer: Buffer },
    buffer: { Buffer: Buffer },
    "node:path": pathShim,
    path: pathShim,
    "node:util": utilShim,
    util: utilShim,
    semver: semver,
    "node:fs": {
      existsSync: exists,
      readFileSync: function (p) { return __readFile(p); },
    },
    fs: {
      existsSync: exists,
      readFileSync: function (p) { return __readFile(p); },
    },
  };

  function packagePath(spec) {
    var prefix = "zigbee-herdsman-converters";
    if (spec === prefix) return join(root, "dist/index.js");
    if (spec.indexOf(prefix + "/") === 0) return join(root, "dist", spec.slice(prefix.length + 1));
    var herdsman = "zigbee-herdsman";
    if (spec.indexOf(herdsman + "/") === 0) return join(herdsmanRoot, spec.slice(herdsman.length + 1));
    return null;
  }

  function makeRequire(fromDir) {
    return function require(spec) {
      if (builtins[spec]) return builtins[spec];
      var mapped = packagePath(spec);
      if (mapped) {
        var resolvedPackage = resolveFile(mapped);
        if (!resolvedPackage) throw new Error("Cannot resolve package path '" + spec + "' (" + mapped + ")");
        return load(resolvedPackage);
      }
      var target;
      if (spec.charAt(0) === ".") target = resolveFile(join(fromDir, spec));
      else if (spec.charAt(0) === "/") target = resolveFile(spec);
      else target = resolveFile(join(root, "node_modules", spec));
      if (!target) throw new Error("Cannot find module '" + spec + "' from '" + fromDir + "'");
      return load(target);
    };
  }

  function load(path) {
    if (moduleCache[path]) return moduleCache[path].exports;
    var module = { exports: {} };
    moduleCache[path] = module;
    if (path.slice(-5) === ".json") {
      module.exports = JSON.parse(__readFile(path));
      return module.exports;
    }
    var source = __readFile(path);
    var factory = new Function("exports", "require", "module", "__filename", "__dirname", source);
    factory(module.exports, makeRequire(dirname(path)), module, path, dirname(path));
    return module.exports;
  }

  // ---- Node shims -------------------------------------------------------
  globalThis.global = globalThis;
  globalThis.console = {
    log: function () {}, info: function () {}, warn: function () {}, error: function () {},
    debug: function () {},
  };
  globalThis.process = {
    env: {}, version: "v22.0.0", platform: "linux", arch: "aarch64",
    nextTick: function (fn) { Promise.resolve().then(fn); },
  };
  globalThis.Buffer = Buffer;
  globalThis.TextEncoder = TextEncoderImpl;
  globalThis.TextDecoder = TextDecoderImpl;
  function makeTimer(id, refed) {
    var timer = { id: id, _refed: refed,
      unref: function () { __unrefTimer(id); this._refed = false; return this; },
      ref: function () { __refTimer(id); this._refed = true; return this; },
      hasRef: function () { return this._refed; },
      close: function () { __clearTimer(id); this._refed = false; },
      [Symbol.toPrimitive]: function () { return id; } };
    return timer;
  }
  function setHostTimer(callback, delay, repeat, args) {
    var callArgs = [callback, Number(delay) || 0, !!repeat, globalThis.__timerJobId || 0,
      globalThis.__timerDevice || ""].concat(args || []);
    var id = __setTimer.apply(null, callArgs);
    return makeTimer(id, true);
  }
  globalThis.setTimeout = function (callback, delay) {
    return setHostTimer(callback, delay, false, Array.prototype.slice.call(arguments, 2));
  };
  globalThis.clearTimeout = function (timer) {
    var id = timer && typeof timer === "object" ? timer.id : timer;
    __clearTimer(Number(id) || 0);
  };
  globalThis.setInterval = function (callback, delay) {
    return setHostTimer(callback, delay, true, Array.prototype.slice.call(arguments, 2));
  };
  globalThis.clearInterval = globalThis.clearTimeout;
  globalThis.queueMicrotask = function (fn) { Promise.resolve().then(fn); };

  var RealDate = Date;
  var fixedNow = RealDate.UTC(2026, 10, 20, 12, 0, 0);
  class FixedDate extends RealDate {
    constructor() {
      if (arguments.length === 0) super(fixedNow); else super(...arguments);
    }
    static now() { return fixedNow; }
  }
  globalThis.Date = FixedDate;

  var pkg = load(join(root, "dist/index.js"));
  globalThis.__pkg = pkg;

  // ---- host-side device mocks ------------------------------------------
  var callLog = [];
  var readValues = {};
  function cloneRecord(value) {
    return value === undefined ? null : JSON.parse(JSON.stringify(value));
  }
  var hostMs = 0;
  var hostCalls = 0;
  var profileHost = !!globalThis.__profileHost;
  var perf = { serializeMs: 0, boundaryMs: 0, resumeMs: 0, mockMs: 0, hostCalls: 0 };
  function profileStart() { return profileHost ? __now() : 0; }
  function profileEnd(started) { if (profileHost) perf.mockMs += __now() - started; }
  function hostCall(op, jobId, jobGeneration, state) {
    var hostOp = jobId === undefined ? op : Object.assign({}, op, { jobId: jobId, generation: jobGeneration });
    var started = __now();
    var text = JSON.stringify(hostOp);
    var serialized = __now();
    var result = __hostOp(text);
    var returned = __now();
    state.hostMs += returned - started;
    state.hostCalls += 1;
    state.perf.serializeMs += serialized - started;
    state.perf.boundaryMs += returned - serialized;
    state.perf.hostCalls += 1;
    if (profileHost && result && typeof result.then === "function") {
      result = result.then(function (value) {
        state.perf.resumeMs += __now() - returned;
        return value;
      });
    }
    return result && typeof result.then === "function" ? result : Promise.resolve(result);
  }

  function makeEndpoint(spec, device, jobId, jobGeneration, state, useHostReadResult) {
    var endpoint = {
      constructor: { name: "Endpoint" },
      ID: spec.id,
      deviceIeeeAddress: device.ieeeAddr,
      deviceNetworkAddress: device.networkAddress,
      inputClusters: spec.inputClusters || [],
      outputClusters: spec.outputClusters || [],
      meta: {},
      binds: [],
      configuredReportings: [],
      device: device,
      getDevice: function () { return device; },
      getCluster: function (key, manufacturerCode) {
        state.perf.endpointGetCluster = (state.perf.endpointGetCluster || 0) + 1;
        return zcl.Utils.getCluster(key, manufacturerCode, device.customClusters);
      },
      inputClusterCache: null,
      outputClusterCache: null,
      getInputClusters: function () {
        state.perf.getInputClusters = (state.perf.getInputClusters || 0) + 1;
        if (endpoint.inputClusterCache) return endpoint.inputClusterCache;
        endpoint.inputClusterCache = endpoint.inputClusters.map(function (key) { return zcl.Utils.getCluster(key); });
        return endpoint.inputClusterCache;
      },
      getOutputClusters: function () {
        if (endpoint.outputClusterCache) return endpoint.outputClusterCache;
        endpoint.outputClusterCache = endpoint.outputClusters.map(function (key) { return zcl.Utils.getCluster(key); });
        return endpoint.outputClusterCache;
      },
      supportsInputCluster: function (key) {
        state.perf.supportsInputCluster = (state.perf.supportsInputCluster || 0) + 1;
        var name = zcl.Utils.getCluster(key).name;
        return endpoint.inputClusters.some(function (candidate) {
          return zcl.Utils.getCluster(candidate).name === name;
        });
      },
      getClusterAttributeValue: function (cluster, attribute) {
        return state.readValues[String(cluster) + ":" + attribute];
      },
      read: function (cluster, attributes) {
        var names = attributes || [];
        var op = { op: "read", endpoint: endpoint.ID, cluster: String(cluster), attributes: names.slice() };
        state.calls.push(op);
        var result = {};
        for (var i = 0; i < names.length; i++) {
          result[names[i]] = state.readValues[String(cluster) + ":" + names[i]] ?? state.readValues[names[i]] ?? 0;
        }
        return hostCall(op, jobId, jobGeneration, state).then(function (response) {
          if (useHostReadResult && response && typeof response === "object") return response;
          return result;
        });
      },
      write: function (cluster, attributes) {
        var op = { op: "write", endpoint: endpoint.ID, cluster: String(cluster), attributes: cloneRecord(attributes || {}) };
        state.calls.push(op);
        return hostCall(op, jobId, jobGeneration, state);
      },
      command: function (cluster, command, payload) {
        var op = { op: "command", endpoint: endpoint.ID, cluster: String(cluster), command: String(command), payload: cloneRecord(payload || {}) };
        state.calls.push(op);
        return hostCall(op, jobId, jobGeneration, state);
      },
      commandResponse: function (cluster, command, payload) {
        var op = { op: "commandResponse", endpoint: endpoint.ID, cluster: String(cluster), command: String(command), payload: cloneRecord(payload || {}) };
        state.calls.push(op);
        return hostCall(op, jobId, jobGeneration, state).then(function () { return { status: 0 }; });
      },
      defaultResponse: function () { return Promise.resolve(); },
      bind: function (cluster, target) {
        var op = { op: "bind", endpoint: endpoint.ID, cluster: String(cluster), target: target && target.ID };
        state.calls.push(op);
        endpoint.binds.push({ cluster: cluster, target: target });
        return hostCall(op, jobId, jobGeneration, state);
      },
      configureReporting: function (cluster, items) {
        var op = { op: "configureReporting", endpoint: endpoint.ID, cluster: String(cluster), items: cloneRecord(items) };
        state.calls.push(op);
        endpoint.configuredReportings.push({ cluster: cluster, items: items });
        return hostCall(op, jobId, jobGeneration, state);
      },
      readResponse: function () { return Promise.resolve(); },
    };
    return endpoint;
  }

  function makeDevice(spec, state) {
    var jobId = spec.__jobId;
    var jobGeneration = spec.__jobGeneration;
    var deviceSpec = spec.device || {};
    var endpointsSpec = deviceSpec.endpoints || [{ id: 1, inputClusters: [], outputClusters: [] }];
    var device = {
      constructor: { name: "Device" },
      ieeeAddr: deviceSpec.ieeeAddr || "0x00124b0000000001",
      networkAddress: deviceSpec.networkAddress || 0x1234,
      modelID: deviceSpec.modelID || spec.definition.model,
      manufacturerName: deviceSpec.manufacturerName || "Bench",
      manufacturerID: deviceSpec.manufacturerID || 0x1234,
      type: deviceSpec.type || "Router",
      meta: deviceSpec.meta || {},
      genBasic: deviceSpec.genBasic || {},
      customClusters: {},
      customReadResponse: undefined,
      endpoints: [],
      getEndpoint: function (id) {
        state.perf.getEndpoint = (state.perf.getEndpoint || 0) + 1;
        for (var i = 0; i < device.endpoints.length; i++) {
          if (device.endpoints[i].ID === id) return device.endpoints[i];
        }
        return undefined;
      },
      addCustomCluster: function (name, definition) {
        state.perf.addCustomCluster = (state.perf.addCustomCluster || 0) + 1;
        state.calls.push({ op: "addCustomCluster", name: name });
        device.customClusters[name] = definition;
        for (var e = 0; e < device.endpoints.length; e++) {
          device.endpoints[e].inputClusterCache = null;
          device.endpoints[e].outputClusterCache = null;
        }
      },
      save: function () { return Promise.resolve(); },
    };
    device.endpoints = endpointsSpec.map(function (endpointSpec) {
      return makeEndpoint(endpointSpec, device, jobId, jobGeneration, state, spec.__useHostReadResult === true);
    });
    return device;
  }

  function findDefinition(spec) {
    var key = spec.source + "|" + (spec.file || "") + "|" + spec.model;
    if (definitionCache[key]) return definitionCache[key];
    var coordinate = null;
    var definitions;
    if (spec.source !== "external") {
      modelsIndex = modelsIndex || JSON.parse(__readFile(join(root, "dist/models-index.json")));
      coordinate = modelsIndex[String(spec.model || "").trim().toLowerCase()];
    }
    if (spec.source === "external") {
      definitions = load(externalPath);
      if (definitions && definitions.default) definitions = definitions.default;
      if (!definitions.definitions) definitions = Array.isArray(definitions) ? definitions : [definitions];
      else definitions = definitions.definitions;
    } else {
      if (!spec.file && coordinate && coordinate[0] && coordinate[0][0]) spec.file = coordinate[0][0];
      var module = load(join(root, "dist/devices", spec.file));
      definitions = module.definitions || module.default || module;
    }
    var normalized = String(spec.model || "").trim().toLowerCase();
    var raw = null;
    if (spec.index !== undefined) raw = definitions[spec.index];
    if (!raw && coordinate && spec.source !== "external" && (!spec.file || coordinate.some(function (entry) { return entry[0] === spec.file; }))) {
      var chosen = coordinate.find(function (entry) { return !spec.file || entry[0] === spec.file; }) || coordinate[0];
      var coordinateDefinitions = load(join(root, "dist/devices", chosen[0]));
      coordinateDefinitions = coordinateDefinitions.definitions || coordinateDefinitions.default || coordinateDefinitions;
      raw = coordinateDefinitions[chosen[1]];
    }
    if (!raw) {
      raw = definitions.find(function (definition) {
        return String(definition.model || "").trim().toLowerCase() === normalized;
      });
    }
    if (!raw) {
      raw = definitions.find(function (definition) {
        var zigbeeModels = Array.isArray(definition.zigbeeModel) ? definition.zigbeeModel : [definition.zigbeeModel];
        return zigbeeModels.some(function (model) {
          return String(model || "").trim().toLowerCase() === normalized;
        });
      });
    }
    if (!raw) throw new Error("definition not found: " + spec.model + " in " + spec.file);
    definitionCache[key] = raw;
    return raw;
  }

  function prepare(spec) {
    var key = spec.source + "|" + (spec.file || "") + "|" + spec.model;
    if (!preparedCache[key]) preparedCache[key] = pkg.prepareDefinition(findDefinition(spec));
    return preparedCache[key];
  }

  async function runScenario(scenario) {
    globalThis.__timerJobId = scenario.__jobId || 0;
    globalThis.__timerDevice = scenario.__deviceKey || (scenario.device && scenario.device.ieeeAddr) || "";
    if (scenario.kind === "jsLoop") {
      while (true) {}
    }
    if (scenario.kind === "read-file") {
      return { content: __readFile(scenario.path) };
    }
    if (scenario.kind === "host-op") {
      var hostOperation = {
        op: "probe",
        jobId: scenario.__jobId || 0,
        generation: scenario.__jobGeneration || 0,
        payload: scenario.payload || "",
      };
      return await new Promise(function (resolve, reject) {
        __hostOp(JSON.stringify(hostOperation))
          .then(function (value) { resolve({ value: value }); })
          .catch(function (error) { reject(error); });
      });
    }
    if (scenario.kind === "timer-error") {
      setTimeout(function () { throw new Error("timer boom"); }, Number(scenario.delay || 20));
      return { scheduled: true };
    }
    if (scenario.kind === "timer-late") {
      globalThis.__lateTimerFired = false;
      setTimeout(function () { globalThis.__lateTimerFired = true; }, Number(scenario.delay || 20));
      return { scheduled: true };
    }
    if (scenario.kind === "timer-check") {
      return { lateTimerFired: globalThis.__lateTimerFired === true };
    }
    if (scenario.kind === "on-event-timer-check") {
      return {
        onEventTimerFired: globalThis.__onEventTimerFired === true,
        onEventWaitTimerFired: globalThis.__onEventWaitTimerFired === true,
      };
    }
    if (scenario.kind === "timer") {
      var timerDelay = Number(scenario.delay || 10);
      return await new Promise(function (resolve) {
        var timer = setTimeout(function () { resolve({ fired: true }); }, timerDelay);
        if (scenario.unref) timer.unref();
        if (scenario.clear) {
          clearTimeout(timer);
          setTimeout(function () { resolve({ fired: false, cleared: true }); }, timerDelay + 20);
        }
      });
    }
    var definition = prepare(scenario.definition);
    var jobState = {
      calls: [],
      readValues: Object.assign({}, scenario.readValues || {}),
      hostMs: 0,
      hostCalls: 0,
      perf: { serializeMs: 0, boundaryMs: 0, resumeMs: 0, mockMs: 0, hostCalls: 0 },
    };
    var device = makeDevice(scenario, jobState);
    var endpointId = scenario.endpointId || 1;
    var endpoint = device.getEndpoint(endpointId) || device.endpoints[0];
    var state = Object.assign({}, scenario.state || {});
    var options = Object.assign({}, scenario.options || {});
    var published = [];
    var converterMs = 0;

    function logger() { return { debug: function () {}, info: function () {}, warn: function () {}, error: function () {} }; }
    var meta = {
      device: device,
      logger: logger(),
      state: Object.assign({}, state),
      options: Object.assign({}, options),
      message: Object.assign({}, scenario.message || {}),
      membersState: {},
      mapped: definition,
      endpoint_name: scenario.endpointName,
      deviceExposesChanged: function () {},
      publish: function (payload) { published.push(payload); },
    };

    async function timed(fn) {
      var started = __now();
      try { return await fn(); } finally { converterMs += __now() - started; }
    }

    if (scenario.kind === "prepare") {
      if (scenario.__metadata !== true) {
        return { result: null, published: [], calls: [], converterMs: converterMs, hostMs: jobState.hostMs, hostCalls: jobState.hostCalls, perf: jobState.perf };
      }
      var zigbeeModels = Array.isArray(definition.zigbeeModel)
        ? definition.zigbeeModel
        : (definition.zigbeeModel ? [definition.zigbeeModel] : []);
      var metadata = {
        model: definition.model || "",
        vendor: definition.vendor || "",
        description: definition.description || "",
        zigbeeModel: zigbeeModels,
        endpoint: definition.endpoint || {},
        version: definition.version || "0.0.0",
        fromZigbee: definition.fromZigbee.map(function (converter) {
          return {
            cluster: converter.cluster,
            type: Array.isArray(converter.type) ? converter.type : [converter.type],
          };
        }),
        toZigbee: definition.toZigbee.map(function (converter) {
          return {
            key: Array.isArray(converter.key) ? converter.key : [converter.key],
            endpoints: Array.isArray(converter.endpoints) ? converter.endpoints : null,
          };
        }),
        hasConfigure: typeof definition.configure === "function",
        hasOnEvent: typeof definition.onEvent === "function",
      };
      return { result: metadata, published: [], calls: [], converterMs: converterMs, hostMs: jobState.hostMs, hostCalls: jobState.hostCalls, perf: jobState.perf };
    }

    if (scenario.kind === "fromZigbee") {
      var type = (scenario.msg && scenario.msg.type) || "attributeReport";
      var converters = definition.fromZigbee.filter(function (candidate) {
        var types = Array.isArray(candidate.type) ? candidate.type : [candidate.type];
        return candidate.cluster === scenario.msg.cluster && types.indexOf(type) >= 0;
      });
      if (!converters.length) throw new Error("no fromZigbee converter for " + scenario.msg.cluster + "/" + type);
      var msg = Object.assign({}, scenario.msg);
      msg.device = device;
      msg.endpoint = endpoint;
      msg.meta = Object.assign({}, scenario.msg.meta || {});
      msg.meta.zclTransactionSequenceNumber = (msg.meta.zclTransactionSequenceNumber || 1) + (scenario.__iteration || 0);
      var convertMeta = { device: device, logger: logger(), state: Object.assign({}, state), deviceExposesChanged: function () {} };
      var merged = {};
      for (var i = 0; i < converters.length; i++) {
        var converted = await timed(function () {
          return converters[i].convert(definition, msg, function (payload) {
            pkg.postProcessConvertedFromZigbeeMessage(definition, payload, options, device);
            published.push(payload);
          }, options, convertMeta);
        });
        if (converted) merged = Object.assign(merged, converted);
      }
      if (Object.keys(merged).length) published.push(merged);
      return { result: Object.keys(merged).length ? merged : null, published: published, calls: jobState.calls.slice(), converterMs: converterMs, hostMs: jobState.hostMs, hostCalls: jobState.hostCalls, perf: jobState.perf };
    }

    if (scenario.kind === "set" || scenario.kind === "get") {
      var toZigbee = definition.toZigbee.filter(function (candidate) {
        var keys = Array.isArray(candidate.key) ? candidate.key : [candidate.key];
        return keys.indexOf(scenario.key) >= 0;
      });
      var converter = toZigbee.find(function (candidate) { return candidate.key !== undefined; })
        || definition.toZigbee.find(function (candidate) { return candidate.key === undefined; });
      if (!converter) throw new Error("no toZigbee converter for " + scenario.key);
      var result = await timed(function () {
        return scenario.kind === "set"
          ? converter.convertSet(endpoint, scenario.key, scenario.value, meta)
          : converter.convertGet(endpoint, scenario.key, meta);
      });
      return { result: result === undefined ? null : result, published: published, calls: jobState.calls.slice(), converterKey: converter.key, converterMs: converterMs, hostMs: jobState.hostMs, hostCalls: jobState.hostCalls, perf: jobState.perf };
    }

    if (scenario.kind === "configure") {
      if (!definition.configure) throw new Error("no configure for " + definition.model);
      var coordinator = {
        ieeeAddr: "0x00124b0000000000",
        networkAddress: 0,
        getEndpoint: function (id) { return makeEndpoint({ id: id, inputClusters: [], outputClusters: [] }, coordinator, scenario.__jobId, scenario.__jobGeneration, jobState, false); },
      };
      coordinator.endpoints = [makeEndpoint({ id: 1, inputClusters: [], outputClusters: [] }, coordinator, scenario.__jobId, scenario.__jobGeneration, jobState, false)];
      coordinator.endpoints[0].deviceIeeeAddress = coordinator.ieeeAddr;
      await timed(function () { return definition.configure(device, coordinator.endpoints[0], definition); });
      return { result: null, published: published, calls: jobState.calls.slice(), converterMs: converterMs, hostMs: jobState.hostMs, hostCalls: jobState.hostCalls, perf: jobState.perf };
    }

    if (scenario.kind === "onEvent") {
      if (!definition.onEvent) throw new Error("no onEvent for " + definition.model);
      var event = { type: scenario.eventType, data: { device: device, state: Object.assign({}, state), options: Object.assign({}, options), deviceExposesChanged: function () {} } };
      if (scenario.eventType === "deviceInterview") event.data.status = "successful";
      await timed(function () { return definition.onEvent(event); });
      return { result: null, published: published, calls: jobState.calls.slice(), converterMs: converterMs, hostMs: jobState.hostMs, hostCalls: jobState.hostCalls, perf: jobState.perf };
    }

    throw new Error("unknown scenario kind " + scenario.kind);
  }

  globalThis.__runScenario = async function (json) {
    try {
      var result = await runScenario(JSON.parse(json));
      globalThis.__lastResult = JSON.stringify(result);
    } catch (error) {
      globalThis.__lastError = String(error && (error.name ? error.name + ": " + error.message + "\n" + (error.stack || "") : error.stack || error));
    }
  };

  globalThis.__bench = async function (json, iterations) {
    var results = [];
    try {
      var scenario = JSON.parse(json);
      for (var i = 0; i < iterations; i++) {
        var started = __now();
        var result = await runScenario(scenario);
        result.wallMs = __now() - started;
        results.push(result);
      }
      globalThis.__lastResult = JSON.stringify(results);
      return results.length;
    } catch (error) {
      globalThis.__lastError = String(error && (error.name ? error.name + ": " + error.message + "\n" + (error.stack || "") : error.stack || error));
      return -1;
    }
  };

  globalThis.__startRuntimeJob = function (id, generation, json) {
    var runtimeSpec = JSON.parse(json);
    runtimeSpec.__jobId = Number(id);
    runtimeSpec.__jobGeneration = Number(generation);
    runScenario(runtimeSpec)
      .then(function (result) { __runtimeDone(String(id), JSON.stringify(result)); })
      .catch(function (error) {
        var text = error && (error.message && error.stack ? error.message + "\n" + error.stack : (error.stack || error.message));
        __runtimeFailed(String(id), String(text || error));
      });
  };

  globalThis.__asyncStart = function (configureJson, jobJson, jobs) {
    var state = { start: __now(), configureDone: null, configureError: null, jobsDone: 0, firstJobDone: null, allDone: null };
    globalThis.__asyncState = state;
    var configurePromise = runScenario(JSON.parse(configureJson))
      .then(function (result) { state.configureResult = result; state.configureDone = __now() - state.start; })
      .catch(function (error) { state.configureError = String(error && error.stack ? error.stack : error); state.configureDone = __now() - state.start; });
    var jobPromises = [];
    for (var i = 0; i < jobs; i++) {
      var jobSpec = JSON.parse(jobJson);
      jobSpec.device = Object.assign({}, jobSpec.device || {}, {
        ieeeAddr: "0x00124b" + (0x2000 + i).toString(16).padStart(10, "0").slice(-10),
        networkAddress: 0x2000 + i,
      });
      jobPromises.push(runScenario(jobSpec).then(function () {
        state.jobsDone += 1;
        if (state.firstJobDone === null) state.firstJobDone = __now() - state.start;
      }).catch(function (error) { state.jobError = String(error && error.stack ? error.stack : error); }));
    }
    Promise.all([configurePromise].concat(jobPromises)).then(function () {
      state.allDone = __now() - state.start;
      globalThis.__lastResult = JSON.stringify(state);
    });
    return true;
  };

  globalThis.__hostErrorProbe = async function () {
    try {
      await __hostOp("{}");
      globalThis.__lastResult = "resolved";
    } catch (error) {
      globalThis.__lastResult = "rejected:" + String(error);
    }
  };

  globalThis.__nowCost = function (n) {
    var started = __now();
    for (var i = 0; i < n; i++) __now();
    return (__now() - started) / n;
  };

  globalThis.__prepareOnly = function (spec) {
    prepare(spec);
    return true;
  };

  globalThis.__init = function () {
    // Force package root load so cold-start includes index + shared modules.
    return pkg !== null;
  };
  globalThis.__ready = true;
})();
