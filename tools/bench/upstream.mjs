// Loads the pinned upstream converter package for the benchmark.
//
// Nothing here reimplements converter behavior: definitions and converters
// come from the installed `zigbee-herdsman-converters` dist.

import { createRequire, Module } from "node:module";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export function createUpstream({ packageRoot, externalPath }) {
  const root = resolve(packageRoot);
  const parent = dirname(root);
  process.env.NODE_PATH = [parent, process.env.NODE_PATH].filter(Boolean).join(":");
  Module._initPaths();

  const packageRequire = createRequire(join(root, "package.json"));
  let pkg = null;
  const packageJson = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const index = JSON.parse(readFileSync(join(root, "dist/models-index.json"), "utf8"));
  const fileCache = new Map();
  const definitionCache = new Map();
  const loadTimes = new Map();
  let rootLoadMs = null;
  let zcl = null;

  function loadRoot() {
    if (pkg === null) {
      const started = performance.now();
      zcl = packageRequire("zigbee-herdsman").Zcl;
      pkg = packageRequire(join(root, "dist/index.js"));
      pkg.setLogger?.({ debug() {}, info() {}, warn() {}, error() {} });
      rootLoadMs = performance.now() - started;
    }
    return rootLoadMs;
  }

  function loadFile(file) {
    if (!fileCache.has(file)) {
      const started = performance.now();
      const module = packageRequire(join(root, "dist/devices", file));
      const definitions = module.definitions ?? [];
      fileCache.set(file, definitions);
      loadTimes.set(file, performance.now() - started);
    }
    return fileCache.get(file);
  }

  function findInDefinitions(definitions, model) {
    if (!model) return null;
    const normalized = normalizeModel(model);
    return (
      definitions.find((definition) => normalizeModel(definition.model) === normalized) ??
      definitions.find((definition) =>
        (Array.isArray(definition.zigbeeModel) ? definition.zigbeeModel : [definition.zigbeeModel])
          .filter(Boolean)
          .some((zigbeeModel) => normalizeModel(zigbeeModel) === normalized),
      ) ??
      null
    );
  }

  function coordinateForModel(model, file) {
    const candidates = index[normalizeModel(model)] ?? [];
    if (file) return candidates.find((candidate) => candidate[0] === file) ?? null;
    return candidates[0] ?? null;
  }

  function loadExternal() {
    if (!externalPath) throw new Error("external converter path not configured");
    loadRoot();
    const started = performance.now();
    const module = createRequire(externalPath)(externalPath);
    const exported = module?.definitions ?? module?.default ?? module;
    const definitions = Array.isArray(exported) ? exported : [exported];
    const elapsed = performance.now() - started;
    return { definitions, loadMs: elapsed, path: externalPath };
  }

  /** Return the raw definition selected by {model,file,index}. */
  function selectRaw({ model, file, index, source = "package" }) {
    if (source === "external") {
      const external = loadExternal();
      const definition = findInDefinitions(external.definitions, model) ?? external.definitions[0];
      return { raw: definition, file: external.path, loadMs: external.loadMs, source };
    }
    const coordinate = coordinateForModel(model, file);
    const selectedFile = file ?? coordinate?.[0];
    const selectedIndex = index ?? coordinate?.[1];
    if (!selectedFile) throw new Error(`model '${model}' not found in models-index`);
    const definitions = loadFile(selectedFile);
    const raw =
      selectedIndex !== undefined
        ? definitions[selectedIndex]
        : findInDefinitions(definitions, model);
    if (!raw) throw new Error(`definition '${model}' not found in ${selectedFile}`);
    return {
      raw,
      file: selectedFile,
      index: definitions.indexOf(raw),
      loadMs: loadTimes.get(selectedFile) ?? 0,
      source,
    };
  }

  function prepare(spec) {
    loadRoot();
    const selected = selectRaw(spec);
    if (selected.source === "external") {
      selected.raw.externalConverterName = selected.file.split("/").pop();
    }
    const started = performance.now();
    const definition = pkg.prepareDefinition(selected.raw);
    const prepareMs = performance.now() - started;
    return {
      definition,
      raw: selected.raw,
      file: selected.file,
      index: selected.index,
      source: selected.source,
      loadMs: selected.loadMs,
      prepareMs,
    };
  }

  function clearStore() {
    pkg?.clearGlobalStore?.();
  }

  return {
    root,
    packageJson,
    index,
    get pkg() {
      loadRoot();
      return pkg;
    },
    prepareRaw(raw) {
      loadRoot();
      return pkg.prepareDefinition(raw);
    },
    zcl: () => {
      loadRoot();
      return zcl;
    },
    loadRoot,
    loadFile,
    selectRaw,
    prepare,
    clearStore,
    fileLoadTimes: loadTimes,
    get rootLoadMs() {
      return rootLoadMs;
    },
    definitionCache,
  };
}

export function normalizeModel(model) {
  return String(model ?? "").trim().toLowerCase();
}
