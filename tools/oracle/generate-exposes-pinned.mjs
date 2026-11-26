// Resolved exposes for a pinned IKEA definition (light features/HA fields).
import {createRequire} from "node:module";
import {writeFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
const require = createRequire(import.meta.url);
const root = fileURLToPath(new URL("../../.upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/", import.meta.url));
const z2m = require(root + "index.js");
const modelsIndex = require(root + "models-index.json");
const model = "TRADFRI bulb E27 WS opal 1000lm";
const coordinate = modelsIndex[model.toLowerCase()];
const mod = require(root + "devices/" + coordinate[0][0]);
const definitions = mod.definitions || mod.default || mod;
const prepared = z2m.prepareDefinition(definitions[coordinate[0][1]]);
const device = {modelID: model, manufacturerName: "IKEA of Sweden", endpoints: [], getEndpoint: () => undefined};
const exposes = typeof prepared.exposes === "function" ? prepared.exposes(device, {}) : prepared.exposes;
writeFileSync(fileURLToPath(new URL("../../crates/z2m-rs/tests/fixtures/exposes-ikea-pinned.json", import.meta.url)), JSON.stringify({revision: "zigbee-herdsman-converters 26.125.0", model, exposes}, null, 2) + "\n");
console.log(JSON.stringify({model: prepared.model, exposes: exposes.length}));
